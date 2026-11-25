mod unity_yaml {
    use std::path::Path;

    use unitas_asset::unity_yaml::{self as yaml, Node, YamlError};

    const SCENE_FIXTURE: &str = "tests/fixtures/sample.unity";
    const PREFAB_FIXTURE: &str = "tests/fixtures/sample.prefab";

    fn missing(text: &str) -> YamlError {
        YamlError::UnexpectedContent {
            line: 0,
            text: text.to_string(),
        }
    }

    #[test]
    fn yaml_header_directives_are_parsed() -> Result<(), YamlError> {
        let text =
            "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &1\nGameObject:\n  m_Name: Cube\n";
        let stream = yaml::parse(text)?;
        let header = match &stream.header {
            Some(header) => header,
            None => return Err(missing("header")),
        };
        assert_eq!(header.yaml_version, "1.1");
        assert_eq!(header.tag_handle, Some(String::from("!u!")));
        assert_eq!(
            header.tag_prefix,
            Some(String::from("tag:unity3d.com,2011:"))
        );
        assert_eq!(stream.documents.len(), 1);
        Ok(())
    }

    #[test]
    fn yaml_document_header_fields_are_parsed() -> Result<(), YamlError> {
        let text = "--- !u!4 &1311639322\nTransform:\n  m_Father: {fileID: 0}\n";
        let stream = yaml::parse(text)?;
        let document = match stream.documents.first() {
            Some(document) => document,
            None => return Err(missing("document")),
        };
        assert_eq!(document.class_id, 4);
        assert_eq!(document.file_id, 1311639322);
        assert!(!document.stripped);
        Ok(())
    }

    #[test]
    fn yaml_stripped_document_is_flagged() -> Result<(), YamlError> {
        let text = "--- !u!114 &42 stripped\nMonoBehaviour:\n  m_Script: {fileID: 11500000}\n";
        let stream = yaml::parse(text)?;
        let document = match stream.documents.first() {
            Some(document) => document,
            None => return Err(missing("document")),
        };
        assert_eq!(document.class_id, 114);
        assert_eq!(document.file_id, 42);
        assert!(document.stripped);
        Ok(())
    }

    #[test]
    fn yaml_negative_file_id_is_parsed() -> Result<(), YamlError> {
        let text = "--- !u!1001 &-8679921383154072067\nPrefab:\n  m_Name: Root\n";
        let stream = yaml::parse(text)?;
        let document = match stream.documents.first() {
            Some(document) => document,
            None => return Err(missing("document")),
        };
        assert_eq!(document.file_id, -8679921383154072067);
        Ok(())
    }

    #[test]
    fn yaml_nested_block_mapping_is_parsed() -> Result<(), YamlError> {
        let text = "--- !u!1 &1\nGameObject:\n  m_Layer: 0\n  m_Name: Main Camera\n  m_Settings:\n    m_Enabled: 1\n";
        let stream = yaml::parse(text)?;
        let document = match stream.documents.first() {
            Some(document) => document,
            None => return Err(missing("document")),
        };
        let game_object = match document.root.get("GameObject") {
            Some(node) => node,
            None => return Err(missing("GameObject")),
        };
        assert_eq!(
            game_object.get("m_Layer").and_then(Node::as_scalar),
            Some("0")
        );
        assert_eq!(
            game_object.get("m_Name").and_then(Node::as_scalar),
            Some("Main Camera")
        );
        let settings = match game_object.get("m_Settings") {
            Some(node) => node,
            None => return Err(missing("m_Settings")),
        };
        assert_eq!(
            settings.get("m_Enabled").and_then(Node::as_scalar),
            Some("1")
        );
        Ok(())
    }

    #[test]
    fn yaml_block_sequence_is_parsed() -> Result<(), YamlError> {
        let text = "--- !u!1 &1\nGameObject:\n  m_Component:\n  - component: {fileID: 1311639322}\n  - component: {fileID: 1311639323}\n";
        let stream = yaml::parse(text)?;
        let document = match stream.documents.first() {
            Some(document) => document,
            None => return Err(missing("document")),
        };
        let components = match document
            .root
            .get("GameObject")
            .and_then(|node| node.get("m_Component"))
            .and_then(Node::as_sequence)
        {
            Some(items) => items,
            None => return Err(missing("m_Component")),
        };
        assert_eq!(components.len(), 2);
        let first = match components.first().and_then(|node| node.get("component")) {
            Some(node) => node,
            None => return Err(missing("component")),
        };
        assert_eq!(
            first.get("fileID").and_then(Node::as_scalar),
            Some("1311639322")
        );
        let second = match components.get(1).and_then(|node| node.get("component")) {
            Some(node) => node,
            None => return Err(missing("component")),
        };
        assert_eq!(
            second.get("fileID").and_then(Node::as_scalar),
            Some("1311639323")
        );
        Ok(())
    }

    #[test]
    fn yaml_flow_collections_are_parsed() -> Result<(), YamlError> {
        let text = "--- !u!4 &1\nTransform:\n  m_LocalRotation: {x: 0, y: 0.5, z: 0}\n  m_Children: []\n  m_Scale: [1, 2, 3]\n";
        let stream = yaml::parse(text)?;
        let document = match stream.documents.first() {
            Some(document) => document,
            None => return Err(missing("document")),
        };
        let transform = match document.root.get("Transform") {
            Some(node) => node,
            None => return Err(missing("Transform")),
        };
        let rotation = match transform.get("m_LocalRotation") {
            Some(node) => node,
            None => return Err(missing("m_LocalRotation")),
        };
        let rotation_entries = match rotation.as_mapping() {
            Some(entries) => entries,
            None => return Err(missing("m_LocalRotation entries")),
        };
        assert_eq!(rotation_entries.len(), 3);
        assert_eq!(rotation.get("y").and_then(Node::as_scalar), Some("0.5"));
        let children = match transform.get("m_Children") {
            Some(node) => node,
            None => return Err(missing("m_Children")),
        };
        assert_eq!(children.as_sequence().map(<[Node]>::len), Some(0));
        let scale = match transform.get("m_Scale") {
            Some(node) => node,
            None => return Err(missing("m_Scale")),
        };
        let scale_items = match scale.as_sequence() {
            Some(items) => items,
            None => return Err(missing("m_Scale items")),
        };
        assert_eq!(scale_items.len(), 3);
        assert_eq!(scale_items[1].as_scalar(), Some("2"));
        Ok(())
    }

    #[test]
    fn yaml_quoted_scalars_are_parsed() -> Result<(), YamlError> {
        let text = "--- !u!1 &1\nGameObject:\n  m_Name: \"Camera (Main)\"\n  m_Tag: 'It''s tagged'\n  m_Note: \"first\\nsecond\"\n";
        let stream = yaml::parse(text)?;
        let document = match stream.documents.first() {
            Some(document) => document,
            None => return Err(missing("document")),
        };
        let game_object = match document.root.get("GameObject") {
            Some(node) => node,
            None => return Err(missing("GameObject")),
        };
        assert_eq!(
            game_object.get("m_Name").and_then(Node::as_scalar),
            Some("Camera (Main)")
        );
        assert_eq!(
            game_object.get("m_Tag").and_then(Node::as_scalar),
            Some("It's tagged")
        );
        assert_eq!(
            game_object.get("m_Note").and_then(Node::as_scalar),
            Some("first\nsecond")
        );
        Ok(())
    }

    #[test]
    fn yaml_scene_fixture_is_parsed() -> Result<(), YamlError> {
        let stream = yaml::parse_file(Path::new(SCENE_FIXTURE))?;
        assert_eq!(stream.documents.len(), 4);
        let game_object = match stream.documents.get(1) {
            Some(document) => document,
            None => return Err(missing("GameObject document")),
        };
        assert_eq!(game_object.class_id, 1);
        assert_eq!(game_object.file_id, 519420021);
        let transform = match stream.documents.get(2) {
            Some(document) => document,
            None => return Err(missing("Transform document")),
        };
        assert_eq!(transform.class_id, 4);
        assert_eq!(transform.file_id, 1311639322);
        let transform_body = match transform.root.get("Transform") {
            Some(node) => node,
            None => return Err(missing("Transform")),
        };
        assert_eq!(
            transform_body
                .get("m_Father")
                .and_then(|node| node.get("fileID"))
                .and_then(Node::as_scalar),
            Some("0")
        );
        assert_eq!(
            transform_body
                .get("m_Children")
                .and_then(Node::as_sequence)
                .map(<[Node]>::len),
            Some(0)
        );
        let child_transform = match stream.documents.get(3) {
            Some(document) => document,
            None => return Err(missing("child transform document")),
        };
        let child_body = match child_transform.root.get("Transform") {
            Some(node) => node,
            None => return Err(missing("Transform")),
        };
        let children = match child_body.get("m_Children").and_then(Node::as_sequence) {
            Some(items) => items,
            None => return Err(missing("m_Children")),
        };
        assert_eq!(children.len(), 1);
        assert_eq!(
            children[0].get("fileID").and_then(Node::as_scalar),
            Some("1311639325")
        );
        let scene_settings = match stream.documents.first() {
            Some(document) => document,
            None => return Err(missing("scene settings document")),
        };
        assert_eq!(scene_settings.class_id, 29);
        let bake = match scene_settings
            .root
            .get("OcclusionCullingSettings")
            .and_then(|node| node.get("m_OcclusionBakeSettings"))
        {
            Some(node) => node,
            None => return Err(missing("m_OcclusionBakeSettings")),
        };
        assert_eq!(
            bake.get("smallestHole").and_then(Node::as_scalar),
            Some("0.25")
        );
        Ok(())
    }

    #[test]
    fn yaml_prefab_fixture_is_parsed() -> Result<(), YamlError> {
        let stream = yaml::parse_file(Path::new(PREFAB_FIXTURE))?;
        assert_eq!(stream.documents.len(), 1);
        let document = match stream.documents.first() {
            Some(document) => document,
            None => return Err(missing("document")),
        };
        assert_eq!(document.class_id, 1);
        assert_eq!(document.file_id, 100100000);
        let modifications = match document
            .root
            .get("PrefabInstance")
            .and_then(|node| node.get("m_Modification"))
            .and_then(|node| node.get("m_Modifications"))
            .and_then(Node::as_sequence)
        {
            Some(items) => items,
            None => return Err(missing("m_Modifications")),
        };
        assert_eq!(modifications.len(), 1);
        let modification = match modifications.first() {
            Some(node) => node,
            None => return Err(missing("modification")),
        };
        assert_eq!(
            modification.get("propertyPath").and_then(Node::as_scalar),
            Some("m_Name")
        );
        assert_eq!(
            modification.get("value").and_then(Node::as_scalar),
            Some("Cube")
        );
        let target = match modification.get("target") {
            Some(node) => node,
            None => return Err(missing("target")),
        };
        assert_eq!(target.get("type").and_then(Node::as_scalar), Some("3"));
        assert_eq!(
            target.get("guid").and_then(Node::as_scalar),
            Some("4e9b7d1f2c3a4b5c6d7e8f9a0b1c2d3e")
        );
        Ok(())
    }

    #[test]
    fn yaml_flow_collection_sequence_item_is_parsed() -> Result<(), YamlError> {
        let text = "--- !u!4 &1\nTransform:\n  m_Children:\n  - {fileID: 1311639323}\n  - {fileID: 1311639324}\n";
        let stream = yaml::parse(text)?;
        let document = match stream.documents.first() {
            Some(document) => document,
            None => return Err(missing("document")),
        };
        let children = match document
            .root
            .get("Transform")
            .and_then(|node| node.get("m_Children"))
            .and_then(Node::as_sequence)
        {
            Some(items) => items,
            None => return Err(missing("m_Children")),
        };
        assert_eq!(children.len(), 2);
        assert_eq!(
            children[0].get("fileID").and_then(Node::as_scalar),
            Some("1311639323")
        );
        assert_eq!(
            children[1].get("fileID").and_then(Node::as_scalar),
            Some("1311639324")
        );
        Ok(())
    }

    #[test]
    fn yaml_plain_scalar_with_colon_and_empty_value_are_parsed() -> Result<(), YamlError> {
        let text =
            "--- !u!1 &1\nGameObject:\n  m_Name: Camera: Main\n  m_EditorClassIdentifier: \n";
        let stream = yaml::parse(text)?;
        let document = match stream.documents.first() {
            Some(document) => document,
            None => return Err(missing("document")),
        };
        let game_object = match document.root.get("GameObject") {
            Some(node) => node,
            None => return Err(missing("GameObject")),
        };
        assert_eq!(
            game_object.get("m_Name").and_then(Node::as_scalar),
            Some("Camera: Main")
        );
        assert_eq!(
            game_object
                .get("m_EditorClassIdentifier")
                .and_then(Node::as_scalar),
            Some("")
        );
        Ok(())
    }

    #[test]
    fn yaml_nested_flow_mapping_is_parsed() -> Result<(), YamlError> {
        let text =
            "--- !u!1001 &1\nPrefabAsset:\n  m_Modification: {m_TransformParent: {fileID: 0}}\n";
        let stream = yaml::parse(text)?;
        let document = match stream.documents.first() {
            Some(document) => document,
            None => return Err(missing("document")),
        };
        let prefab = match document.root.get("PrefabAsset") {
            Some(node) => node,
            None => return Err(missing("PrefabAsset")),
        };
        let modification = match prefab.get("m_Modification") {
            Some(node) => node,
            None => return Err(missing("m_Modification")),
        };
        let parent = match modification.get("m_TransformParent") {
            Some(node) => node,
            None => return Err(missing("m_TransformParent")),
        };
        assert_eq!(parent.get("fileID").and_then(Node::as_scalar), Some("0"));
        Ok(())
    }

    #[test]
    fn yaml_invalid_class_id_is_an_error() -> Result<(), YamlError> {
        let result = yaml::parse("--- !u!x &1\nGameObject:\n  m_Name: Cube\n");
        assert!(matches!(result, Err(YamlError::InvalidClassId { .. })));
        Ok(())
    }

    #[test]
    fn yaml_invalid_file_id_is_an_error() -> Result<(), YamlError> {
        let result = yaml::parse("--- !u!1 &abc\nGameObject:\n  m_Name: Cube\n");
        assert!(matches!(result, Err(YamlError::InvalidFileId { .. })));
        Ok(())
    }

    #[test]
    fn yaml_invalid_document_header_is_an_error() -> Result<(), YamlError> {
        let result = yaml::parse("--- GameObject:\n  m_Name: Cube\n");
        assert!(matches!(
            result,
            Err(YamlError::InvalidDocumentHeader { .. })
        ));
        Ok(())
    }

    #[test]
    fn yaml_unterminated_flow_is_an_error() -> Result<(), YamlError> {
        let result = yaml::parse("--- !u!1 &1\nGameObject:\n  m_Icon: {fileID: 0\n");
        assert!(matches!(result, Err(YamlError::UnterminatedFlow { .. })));
        Ok(())
    }

    #[test]
    fn yaml_unterminated_string_is_an_error() -> Result<(), YamlError> {
        let result = yaml::parse("--- !u!1 &1\nGameObject:\n  m_Name: \"Camera\n");
        assert!(matches!(result, Err(YamlError::UnterminatedString { .. })));
        Ok(())
    }

    #[test]
    fn yaml_tab_indentation_is_an_error() -> Result<(), YamlError> {
        let result = yaml::parse("--- !u!1 &1\nGameObject:\n\tm_Name: Cube\n");
        assert!(matches!(result, Err(YamlError::TabIndentation { .. })));
        Ok(())
    }

    #[test]
    fn yaml_empty_document_body_is_an_error() -> Result<(), YamlError> {
        let result = yaml::parse("--- !u!1 &1\n");
        assert!(matches!(result, Err(YamlError::UnexpectedContent { .. })));
        Ok(())
    }

    #[test]
    fn yaml_node_helpers_extract_typed_values() -> Result<(), YamlError> {
        let scalar_int = Node::Scalar(String::from("42"));
        assert_eq!(scalar_int.as_str(), Some("42"));
        assert_eq!(scalar_int.as_i64(), Some(42));
        assert_eq!(scalar_int.as_f64(), Some(42.0));

        let scalar_float = Node::Scalar(String::from("-2.5"));
        assert_eq!(scalar_float.as_str(), Some("-2.5"));
        assert_eq!(scalar_float.as_i64(), None);
        assert_eq!(scalar_float.as_f64(), Some(-2.5));

        let non_numeric = Node::Scalar(String::from("hello"));
        assert_eq!(non_numeric.as_str(), Some("hello"));
        assert_eq!(non_numeric.as_i64(), None);
        assert_eq!(non_numeric.as_f64(), None);

        let mapping = Node::Mapping(vec![(
            String::from("key"),
            Node::Scalar(String::from("100")),
        )]);
        assert_eq!(mapping.as_str(), None);
        assert_eq!(mapping.as_i64(), None);
        assert_eq!(mapping.as_f64(), None);
        assert!(mapping.as_mapping().is_some());
        assert_eq!(mapping.get("key").and_then(|node| node.as_i64()), Some(100));

        let seq = Node::Sequence(vec![Node::Scalar(String::from("1"))]);
        assert!(seq.as_sequence().is_some());
        assert_eq!(seq.as_str(), None);
        Ok(())
    }

    #[test]
    fn yaml_type_aliases_and_document_helpers_work() -> Result<(), unitas_asset::AssetError> {
        let text = "--- !u!1 &1000\nGameObject:\n  m_Name: Main\n";
        let stream = yaml::parse(text)?;
        let doc: &unitas_asset::Document = match stream.documents.first() {
            Some(doc) => doc,
            None => return Err(missing("document")),
        };
        assert_eq!(doc.file_id_u64(), Some(1000));
        let root: &unitas_asset::YamlNode = &doc.root;
        let name = match root
            .get("GameObject")
            .and_then(|game_object| game_object.get("m_Name"))
        {
            Some(node) => node.as_str(),
            None => None,
        };
        assert_eq!(name, Some("Main"));
        Ok(())
    }
}
